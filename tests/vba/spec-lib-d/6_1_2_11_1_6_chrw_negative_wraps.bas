' INV: vbal.6.1.2.11.1.6
' SOURCE: spec ms-vbal 6.1.2.11.1.6 ChrW code point, negative values get 65536 added
' EXPECT: a=65
' EXPECT: b=65535
' EXPECT: c=-1
' EXPECT: d=32769
' EXPECT: e=65535
' EXPECT: f=String
' EXPECT: g=String
' EXPECT: h=1
Public Sub Main()
    Debug.Print "a=" & CStr(AscW(ChrW(65)))
    Debug.Print "b=" & CStr(AscW(ChrW(65535)) And &HFFFF&)
    Debug.Print "c=" & CStr(AscW(ChrW(-1)))
    Debug.Print "d=" & CStr(AscW(ChrW(-32767)) And &HFFFF&)
    Debug.Print "e=" & CStr(AscW(ChrW(-1)) And &HFFFF&)
    Debug.Print "f=" & TypeName(ChrW(65))
    Debug.Print "g=" & TypeName(ChrW$(65))
    Debug.Print "h=" & CStr(Len(ChrW(&H263A)))
End Sub
