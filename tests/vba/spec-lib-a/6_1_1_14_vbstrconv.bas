' INV: vbal.6.1.1.14
' SOURCE: spec ms-vbal 6.1.1.14 VbStrConv values
' EXPECT: vbFromUnicode=128
' EXPECT: vbHiragana=32
' EXPECT: vbKatakana=16
' EXPECT: vbLowerCase=2
' EXPECT: vbNarrow=8
' EXPECT: vbProperCase=3
' EXPECT: vbUnicode=64
' EXPECT: vbUpperCase=1
' EXPECT: vbWide=4
Public Sub Main()
    Debug.Print "vbFromUnicode=" & CStr(vbFromUnicode)
    Debug.Print "vbHiragana=" & CStr(vbHiragana)
    Debug.Print "vbKatakana=" & CStr(vbKatakana)
    Debug.Print "vbLowerCase=" & CStr(vbLowerCase)
    Debug.Print "vbNarrow=" & CStr(vbNarrow)
    Debug.Print "vbProperCase=" & CStr(vbProperCase)
    Debug.Print "vbUnicode=" & CStr(vbUnicode)
    Debug.Print "vbUpperCase=" & CStr(vbUpperCase)
    Debug.Print "vbWide=" & CStr(vbWide)
End Sub
