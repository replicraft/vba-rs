' INV: decl.paramarray
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/function-statement
' EXPECT:  6 
' EXPECT:  0 
Function Total(ParamArray v() As Variant) As Long
    Dim i As Long, s As Long
    For i = LBound(v) To UBound(v)
        s = s + v(i)
    Next
    Total = s
End Function
Public Sub Main()
    Debug.Print Total(1, 2, 3)
    Debug.Print Total()
End Sub
