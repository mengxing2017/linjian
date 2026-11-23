<script setup lang="ts">
import { computed, onUnmounted, ref, shallowRef } from 'vue'
import { schoolChangesApi, type SchoolChangeDocument, type SchoolChangePolicy, type SchoolChangeRequest } from './api'
import { createSubmissionKeys } from './submission'
import { session, restoreSession, login } from '../identity/session'
import { schoolsApi, type SchoolDisplay } from '../school-context/api'
import { miniappUserApi } from '../miniapp-users/api'
import { credentials } from '../../app/credentials'
import { confirm, usePage } from '../../shared/use-page'
import PageStatus from '../../shared/PageStatus.vue'
import { acceptDocumentUpload } from './document-upload'

const policy = shallowRef<SchoolChangePolicy>()
const request = shallowRef<SchoolChangeRequest | null>(null)
const schools = shallowRef<SchoolDisplay[]>([])
const search = ref(''), page = ref(1), more = ref(false), target = ref('')
const currentSchoolName = ref('')
const studentName = ref(''), studentNumber = ref(''), reason = ref(''), consent = ref(false)
const relationship = ref<'enrolled' | 'new_student' | ''>('')
const documents = shallowRef<Array<SchoolChangeDocument & { size_bytes: number }>>([])
const previews = ref<Record<string, string>>({})
const uploadResult = ref('')
const temporary = new Set<string>()
let submissionKey = createSubmissionKeys()
let visible = true, lifetime = 0
const statuses = { pending: '审核中', approved: '已通过', rejected: '已驳回', cancelled: '已撤回', invalidated: '已失效' }
function clearFiles() {
  for (const path of temporary) uni.getFileSystemManager().unlink({ filePath: path, fail: () => {} })
  temporary.clear()
  previews.value = {}
}
function clear() {
  uploadResult.value = ''
  clearFiles()
  policy.value = undefined
  request.value = null
  schools.value = []
  currentSchoolName.value = ''
  documents.value = []
  target.value = studentName.value = studentNumber.value = reason.value = ''
  consent.value = false
  relationship.value = ''
  submissionKey = createSubmissionKeys()
}
const { busy, error, run } = usePage(clear)
const canApply = computed(() => !!session.user?.bound_school_id && !!policy.value && request.value?.status !== 'pending')
async function load() {
  visible = true
  await run(async current => {
    await restoreSession()
    if (!session.user) return
    const [nextPolicy, nextRequest, profile] = await Promise.all([schoolChangesApi.policy(), schoolChangesApi.current(), miniappUserApi.profile()])
    if (current()) { policy.value = nextPolicy; request.value = nextRequest; currentSchoolName.value = profile.school_name || '尚未绑定学校' }
  })
}
async function signIn() {
  try { await login(); await load() } catch (failure) { error.value = failure instanceof Error ? failure.message : '登录失败' }
}
function findSchools(reset = true) {
  void run(async current => {
    const next = reset ? 1 : page.value + 1
    const result = await schoolsApi.directory({ search: search.value, page: next })
    if (current()) {
      const candidates = result.items.filter(school => school.id !== session.user?.bound_school_id)
      schools.value = reset ? candidates : [...schools.value, ...candidates]
      page.value = next
      more.value = !!result.page?.has_more
    }
  })
}
async function readFile(path: string) {
  return new Promise<ArrayBuffer>((resolve, reject) => uni.getFileSystemManager().readFile({ filePath: path,
    success: result => typeof result.data === 'string' ? reject(new Error('图片读取失败')) : resolve(result.data), fail: reject }))
}
function upload() {
  void run(async current => {
    uploadResult.value = ''
    const turn = lifetime
    if (!policy.value || documents.value.length >= policy.value.max_documents) throw new Error('材料数量已达上限')
    const chosen = await uni.chooseImage({ count: 1 })
    const path = chosen.tempFilePaths[0]
    if (!path) return
    if (!current() || !visible || turn !== lifetime) { uni.getFileSystemManager().unlink({ filePath: path, fail: () => {} }); return }
    temporary.add(path)
    const bytes = await readFile(path)
    if (!current() || !visible || turn !== lifetime) return
    if (bytes.byteLength > policy.value.max_file_bytes) throw new Error('材料超过单张大小限制')
    const result = await schoolChangesApi.upload(bytes)
    if (current() && visible && turn === lifetime) {
      const accepted = acceptDocumentUpload(documents.value, result, policy.value.max_total_bytes)
      documents.value = accepted.documents
      previews.value = { ...previews.value, [result.id]: path }
      uploadResult.value = accepted.message
    }
  })
}
function submit() {
  void run(async current => {
    if (!canApply.value || !policy.value || !consent.value) throw new Error('请阅读并确认处理告知')
    if (!target.value || !studentName.value.trim() || !reason.value.trim() || !documents.value.length) throw new Error('请填写目标学校、姓名、原因并上传证明')
    if (policy.value.student_number_required && !studentNumber.value.trim()) throw new Error('请填写学生编号')
    const studentRelationship = relationship.value
    if (!studentRelationship) throw new Error('请选择目标学校学生关系，仅受理在读或新生')
    const payload = { to_school_id: target.value, student_relationship: studentRelationship, declared_student_name: studentName.value.trim(), declared_student_number: studentNumber.value.trim() || null,
      reason: reason.value.trim(), document_ids: documents.value.map(document => document.id).sort(), consent_version: policy.value.version! }
    const result = await schoolChangesApi.submit({ ...payload, submission_key: submissionKey(payload) })
    if (current()) { request.value = result; documents.value = []; uploadResult.value = ''; studentName.value = studentNumber.value = reason.value = relationship.value = ''; consent.value = false; clearFiles() }
  })
}
function cancel() {
  void run(async current => {
    if (!request.value || !(await confirm('确认撤回待审申请？'))) return
    const result = await schoolChangesApi.cancel(request.value)
    if (current()) request.value = result
  })
}
function preview(id: string) {
  void run(async current => {
    const turn = lifetime
    const revision = credentials.revision
    const bytes = await schoolChangesApi.document(id)
    if (!current() || !visible || turn !== lifetime || revision !== credentials.revision) return
    const path = `${(uni as typeof uni & { env: { USER_DATA_PATH: string } }).env.USER_DATA_PATH}/linjian-school-document-${id}-${Date.now()}.png`
    await new Promise<void>((resolve, reject) => uni.getFileSystemManager().writeFile({ filePath: path, data: bytes, success: () => resolve(), fail: reject }))
    temporary.add(path)
    if (!current() || !visible || turn !== lifetime) { clearFiles(); return }
    uni.previewImage({ urls: [path] })
  })
}
function hide() { visible = false; lifetime++; uploadResult.value = ''; clearFiles() }
onUnmounted(hide)
defineExpose({ show: load, refresh: load, hide })
</script>
<template>
  <view class="page">
    <text class="heading">申请更换学校</text>
    <text class="muted">只变更学校归属，不搬迁历史内容；核验的是您与目标学校的学生关系。</text>
    <button v-if="!session.user" :disabled="busy" @tap="signIn">微信登录并查询结果</button>
    <PageStatus :busy="busy" :error="error" @retry="load" />
    <view v-if="request" class="card">
      <text class="heading">{{ statuses[request.status] }}</text>
      <text>{{ request.from_school_name }} → {{ request.to_school_name }}</text>
      <text class="muted">提交：{{ request.submitted_at }}</text>
      <text>审核截止：{{ request.review_due_at }}；逾期未完成审核将自动驳回。</text>
      <text>声明学生关系：{{ request.student_relationship === 'enrolled' ? '在读' : request.student_relationship === 'new_student' ? '新生' : '旧申请未声明' }}</text>
      <text v-if="request.result_reason_code === 'review_timeout'">因超期未审核被系统自动驳回，并非材料审核不通过，可重新申请。</text>
      <text v-if="request.result_reason">{{ request.result_reason }}</text>
      <text v-if="request.status === 'invalidated'">学校归属已由其他操作变更，请核对当前学校后重新申请。</text>
      <text v-if="request.status === 'approved'">已通过，证明材料已停止访问并进入自动删除流程。当前登录为重新认证后的会话，新学校权限以服务端为准。</text>
      <text v-if="request.personal_data_purged">声明个人信息已按保留策略清理。</text>
      <button v-for="document in request.documents" :key="document.id" :disabled="busy || !document.available" @tap="preview(document.id)">
        {{ document.available ? '查看本人证明材料' : '材料已不可用' }}
      </button>
      <button v-if="request.status === 'pending'" :disabled="busy" @tap="cancel">撤回申请</button>
    </view>
    <text v-if="session.user && !session.user.bound_school_id">尚未绑定学校，请先使用学校绑定入口。</text>
    <view v-if="canApply && policy" class="card">
      <text>当前归属：{{ currentSchoolName }}</text>
      <text>仅受理目标学校在读学生、新生；其他学生关系不予受理。审核期限 {{ policy.review_working_days }} 个工作日，提交次日起按北京时间计算，超期自动驳回。</text>
      <button :disabled="busy" @tap="relationship = 'enrolled'">{{ relationship === 'enrolled' ? '✓ ' : '' }}在读</button>
      <button :disabled="busy" @tap="relationship = 'new_student'">{{ relationship === 'new_student' ? '✓ ' : '' }}新生</button>
      <input v-model="search" class="input" placeholder="搜索目标学校" />
      <button :disabled="busy" @tap="findSchools()">查询正常学校目录</button>
      <text v-if="!schools.length">请搜索目标学校；若目录中没有该学校，请联系平台。</text>
      <button v-for="school in schools" :key="school.id" :disabled="busy" @tap="target = school.id">{{ target === school.id ? '✓ ' : '' }}{{ school.name }}</button>
      <button v-if="more" :disabled="busy" @tap="findSchools(false)">加载更多</button>
      <input v-model="studentName" class="input" maxlength="100" placeholder="证明材料中的姓名" />
      <input v-model="studentNumber" class="input" maxlength="100" :placeholder="policy.student_number_required ? '学生编号（必填）' : '学生编号（按材料需要填写）'" />
      <textarea v-model="reason" class="input" maxlength="500" placeholder="选错学校、转学、升学等原因；勿填写无关个人信息" />
      <text>材料仅用于本次审核，审核通过后自动删除。请提供能证明目标学校在读或新生关系的材料，具体要求见处理告知，可遮盖无关信息。格式 JPEG/PNG/WebP，单张 {{ Math.floor(policy.max_file_bytes / 1024) }} KiB，最多 {{ policy.max_documents }} 张。</text>
      <view v-for="document in documents" :key="document.id">
        <image v-if="previews[document.id]" :src="previews[document.id]" mode="widthFix" style="width: 200rpx" />
        <button :disabled="busy" @tap="documents = documents.filter(item => item.id !== document.id); uploadResult = ''">移除此材料</button>
      </view>
      <button :disabled="busy || documents.length >= policy.max_documents" @tap="upload">上传证明</button>
      <text v-if="uploadResult">{{ uploadResult }}</text>
      <text>{{ policy.notice }}</text>
      <button :disabled="busy" @tap="consent = !consent">{{ consent ? '✓ 已阅读并确认' : '阅读后点击确认处理告知' }}</button>
      <button class="primary" :disabled="busy || !consent" @tap="submit">提交换校申请</button>
    </view>
    <button :disabled="busy" @tap="load">刷新审核结果</button>
  </view>
</template>
