' INV: vbal.6.1.1.10
' SOURCE: spec ms-vbal 6.1.1.10 VbIMEStatus values
' XFAIL: wrong pending
' EXPECT: vbIMEAlphaDbl=7
' EXPECT: vbIMEAlphaSng=8
' EXPECT: vbIMEDisable=3
' EXPECT: vbIMEHiragana=4
' EXPECT: vbIMEKatakanaDbl=5
' EXPECT: vbIMEKatakanaSng=6
' EXPECT: vbIMEModeAlpha=8
' EXPECT: vbIMEModeAlphaFull=7
' EXPECT: vbIMEModeDisable=3
' EXPECT: vbIMEModeHangul=10
' EXPECT: vbIMEModeHangulFull=9
' EXPECT: vbIMEModeHiragana=4
' EXPECT: vbIMEModeKatakana=5
' EXPECT: vbIMEModeKatakanaHalf=6
' EXPECT: vbIMEModeNoControl=0
' EXPECT: vbIMEModeOff=2
' EXPECT: vbIMEModeOn=1
' EXPECT: vbIMENoOp=0
' EXPECT: vbIMEOff=2
' EXPECT: vbIMEOn=1
Public Sub Main()
    Debug.Print "vbIMEAlphaDbl=" & CStr(vbIMEAlphaDbl)
    Debug.Print "vbIMEAlphaSng=" & CStr(vbIMEAlphaSng)
    Debug.Print "vbIMEDisable=" & CStr(vbIMEDisable)
    Debug.Print "vbIMEHiragana=" & CStr(vbIMEHiragana)
    Debug.Print "vbIMEKatakanaDbl=" & CStr(vbIMEKatakanaDbl)
    Debug.Print "vbIMEKatakanaSng=" & CStr(vbIMEKatakanaSng)
    Debug.Print "vbIMEModeAlpha=" & CStr(vbIMEModeAlpha)
    Debug.Print "vbIMEModeAlphaFull=" & CStr(vbIMEModeAlphaFull)
    Debug.Print "vbIMEModeDisable=" & CStr(vbIMEModeDisable)
    Debug.Print "vbIMEModeHangul=" & CStr(vbIMEModeHangul)
    Debug.Print "vbIMEModeHangulFull=" & CStr(vbIMEModeHangulFull)
    Debug.Print "vbIMEModeHiragana=" & CStr(vbIMEModeHiragana)
    Debug.Print "vbIMEModeKatakana=" & CStr(vbIMEModeKatakana)
    Debug.Print "vbIMEModeKatakanaHalf=" & CStr(vbIMEModeKatakanaHalf)
    Debug.Print "vbIMEModeNoControl=" & CStr(vbIMEModeNoControl)
    Debug.Print "vbIMEModeOff=" & CStr(vbIMEModeOff)
    Debug.Print "vbIMEModeOn=" & CStr(vbIMEModeOn)
    Debug.Print "vbIMENoOp=" & CStr(vbIMENoOp)
    Debug.Print "vbIMEOff=" & CStr(vbIMEOff)
    Debug.Print "vbIMEOn=" & CStr(vbIMEOn)
End Sub
