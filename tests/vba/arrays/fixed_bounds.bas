' INV: decl.array-bounds, func.lbound, func.ubound
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/lbound-function
' EXPECT:  0  5 
' EXPECT:  2  4 
' EXPECT: -1  1 
Public Sub Main()
    Dim a(5) As Long, b(2 To 4) As Long, c(-1 To 1) As Long
    Debug.Print LBound(a); UBound(a)
    Debug.Print LBound(b); UBound(b)
    Debug.Print LBound(c); UBound(c)
End Sub
