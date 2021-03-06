#[doc = "Reader of register CCBUF%s"]
pub type R = crate::R<u16, super::CCBUF>;
#[doc = "Writer for register CCBUF%s"]
pub type W = crate::W<u16, super::CCBUF>;
#[doc = "Register CCBUF%s `reset()`'s with value 0"]
impl crate::ResetValue for super::CCBUF {
    type Type = u16;
    #[inline(always)]
    fn reset_value() -> Self::Type {
        0
    }
}
#[doc = "Reader of field `CCBUF`"]
pub type CCBUF_R = crate::R<u16, u16>;
#[doc = "Write proxy for field `CCBUF`"]
pub struct CCBUF_W<'a> {
    w: &'a mut W,
}
impl<'a> CCBUF_W<'a> {
    #[doc = r"Writes raw bits to the field"]
    #[inline(always)]
    pub unsafe fn bits(self, value: u16) -> &'a mut W {
        self.w.bits = (self.w.bits & !0xffff) | ((value as u16) & 0xffff);
        self.w
    }
}
impl R {
    #[doc = "Bits 0:15 - Counter/Compare Buffer Value"]
    #[inline(always)]
    pub fn ccbuf(&self) -> CCBUF_R {
        CCBUF_R::new((self.bits & 0xffff) as u16)
    }
}
impl W {
    #[doc = "Bits 0:15 - Counter/Compare Buffer Value"]
    #[inline(always)]
    pub fn ccbuf(&mut self) -> CCBUF_W {
        CCBUF_W { w: self }
    }
}
