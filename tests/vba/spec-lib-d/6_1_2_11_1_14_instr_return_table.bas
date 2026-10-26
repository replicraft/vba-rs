' INV: vbal.6.1.2.11.1.14, func.instr
' SOURCE: spec ms-vbal 6.1.2.11.1.14 InStr return table (zero-length, Null, not found, found, start beyond)
' EXPECT: a=0
' EXPECT: b=True
' EXPECT: c=3
' EXPECT: d=True
' EXPECT: e=0
' EXPECT: f=3
' EXPECT: g=0
' EXPECT: h=Long
' EXPECT: i=2
' EXPECT: j=4
Public Sub Main()
    Debug.Print "a=" & CStr(InStr("", "a"))
    Debug.Print "b=" & CStr(IsNull(InStr(Null, "a")))
    Debug.Print "c=" & CStr(InStr(3, "abcdef", ""))
    Debug.Print "d=" & CStr(IsNull(InStr("abc", Null)))
    Debug.Print "e=" & CStr(InStr("abc", "z"))
    Debug.Print "f=" & CStr(InStr("abcabc", "c"))
    Debug.Print "g=" & CStr(InStr(10, "abc", "a"))
    Debug.Print "h=" & TypeName(InStr("abc", "b"))
    Debug.Print "i=" & CStr(InStr(1, "abc", "bc"))
    Debug.Print "j=" & CStr(InStr(2, "abcabc", "a"))
End Sub
