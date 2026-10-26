' INV: vbal.5.6.16.4, stmt.redim
' SOURCE: spec ms-vbal 5.6.16.4 integer expression is the expression Let-coerced to Long (dynamic array bounds)
' EXPECT: 4
' EXPECT: 3
' EXPECT: 2
Public Sub Main()
    Dim a() As Long
    ReDim a(0 To 3.6)
    Debug.Print CStr(UBound(a))
    ReDim a(0 To "3")
    Debug.Print CStr(UBound(a))
    ReDim a(0 To CByte(2))
    Debug.Print CStr(UBound(a))
End Sub
