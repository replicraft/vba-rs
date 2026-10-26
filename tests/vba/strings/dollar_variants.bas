' INV: func.left, func.right, func.mid, func.lcase, func.ucase, func.trim, func.space
' SOURCE: spec ms-vbal 6.1.2.11.1.20 string-returning $ variants (Left$ Right$ Mid$ LCase$ UCase$ Trim$ Space$)
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
