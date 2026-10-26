' INV: vbal.6.1.2.11.1.3
' SOURCE: spec ms-vbal 6.1.2.11.1.3 AscW 16-bit code point, values above 32767 are negative
' EXPECT: a=65
' EXPECT: b=-32768
' EXPECT: c=-1
' EXPECT: d=32767
' EXPECT: e=65
Public Sub Main()
    Debug.Print "a=" & CStr(AscW("A"))
    Debug.Print "b=" & CStr(AscW(ChrW(&H8000)))
    Debug.Print "c=" & CStr(AscW(ChrW(65535)))
    Debug.Print "d=" & CStr(AscW(ChrW(32767)))
    Debug.Print "e=" & CStr(AscW("ABC"))
End Sub
