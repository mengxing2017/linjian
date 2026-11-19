import { useEffect, useState } from 'react'
import type { PublicSiteSettings } from './api'
import { useSiteSettings } from './provider'
import { text } from './title'

const ICP_QUERY = 'https://beian.miit.gov.cn/'

export function SiteFooter({ value }: { value?: PublicSiteSettings }) {
  const { settings } = useSiteSettings()
  const current = value ?? settings
  const [showPoliceIcon, setShowPoliceIcon] = useState(true)
  const owner = text(current.copyright_owner)
  const years = text(current.copyright_years)
  const notice = text(current.copyright_notice)
  const icp = text(current.icp_number)
  const police = text(current.police_number)
  const policeUrl = text(current.police_url)
  const policeConfigured = !!police && !!policeUrl
  const copyright = owner && years
  useEffect(() => {
    setShowPoliceIcon(true)
  }, [policeUrl])
  if (!copyright && !icp && !policeConfigured) return null
  return (
    <footer className="site-footer" aria-label="站点备案与版权信息">
      <div className="site-footer-inner">
        {copyright && (
          <div className="site-footer-copyright">
            <span>
              Copyright © {years ? `${years} ` : ''}
              {owner}
            </span>
            {notice && <span>{notice}</span>}
          </div>
        )}
        {icp && (
          <div className="site-footer-group site-footer-icp">
            <span>{icp}</span>
            <a href={ICP_QUERY} target="_blank" rel="noopener noreferrer">
              工信部备案查询
            </a>
          </div>
        )}
        {policeConfigured && (
          <div className="site-footer-group">
            <a href={policeUrl} target="_blank" rel="noopener noreferrer">
              {showPoliceIcon && (
                <img
                  src="/assets/beian-police.png"
                  alt=""
                  width={16}
                  height={16}
                  onError={() => setShowPoliceIcon(false)}
                />
              )}
              <span>{police}</span>
            </a>
          </div>
        )}
      </div>
    </footer>
  )
}
