' INV: vbal.2.3, type.string-fixed
' SOURCE: spec ms-vbal 2.3 variables, a String*n variable starts as n characters U+0000; its value always has length exactly n
' EXPECT: 3
' EXPECT: 0 0 0
' EXPECT: 3
' EXPECT: ab
Public Sub Main()
    Dim s As String * 3
    Debug.Print CStr(Len(s))
    Debug.Print CStr(Asc(Mid(s, 1, 1))) & " " & CStr(Asc(Mid(s, 2, 1))) & " " & CStr(Asc(Mid(s, 3, 1)))
    s = "a"
    Debug.Print CStr(Len(s))
    s = "abcd"
    Debug.Print Left(s, 2)
End Sub
