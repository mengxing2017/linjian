import {expect,it} from 'vitest'
import {codesFromNames,emptyRegion,formatRegion,namesFromCodes} from '@linjian/china-regions'

it('maps standard region names to codes and back',()=>{
 const value={province:'广东省',city:'广州市',area:'天河区'}
 const codes=codesFromNames(value)
 expect(codes).toEqual(['44','4401','440106'])
 expect(namesFromCodes(codes!)).toEqual(value)
})

it('supports partial province or province-city paths',()=>{
 expect(codesFromNames({province:'广东省',city:'',area:''})).toEqual(['44'])
 expect(namesFromCodes(['44'])).toEqual({province:'广东省',city:'',area:''})
 expect(codesFromNames({province:'广东省',city:'广州市',area:''})).toEqual(['44','4401'])
})

it('rejects non-standard or incomplete paths',()=>{
 expect(codesFromNames(emptyRegion)).toEqual([])
 expect(codesFromNames({province:'',city:'广州市',area:''})).toEqual([])
 expect(codesFromNames({province:'不存在省',city:'',area:''})).toBeNull()
 expect(codesFromNames({province:'广东省',city:'不存在市',area:''})).toBeNull()
 expect(codesFromNames({province:'广东省',city:'广州市',area:'不存在区'})).toBeNull()
})

it('formats duplicated municipality levels',()=>{
 expect(formatRegion({province:'北京市',city:'北京市',area:'东城区'})).toBe('北京市 / 东城区')
 expect(formatRegion({province:'广东省',city:'广州市',area:'天河区'})).toBe('广东省 / 广州市 / 天河区')
 expect(formatRegion(emptyRegion)).toBe('')
})
