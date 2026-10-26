' INV: func.left, func.right, func.mid, func.lcase, func.ucase, func.trim, func.space
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT: ab
' EXPECT: bc
' EXPECT: b
' EXPECT: ab
' EXPECT: AB
' EXPECT: a
' EXPECT:   x
Public Sub Main()
    Debug.Print CStr(Left$("abc", 2))
    Debug.Print CStr(Right$("abc", 2))
    Debug.Print CStr(Mid$("abc", 2, 1))
    Debug.Print CStr(LCase$("AB"))
    Debug.Print CStr(UCase$("ab"))
    Debug.Print CStr(Trim$(" a "))
    Debug.Print CStr(Space$(2) & "x")
End Sub
