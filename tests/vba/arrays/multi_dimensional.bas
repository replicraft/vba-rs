' INV: decl.array-bounds, func.lbound, func.ubound
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/lbound-function
' EXPECT:  1  2  0  3 
' EXPECT:  7  0 
Public Sub Main()
    Dim m(1 To 2, 0 To 3) As Long
    m(2, 3) = 7
    Debug.Print LBound(m, 1); UBound(m, 1); LBound(m, 2); UBound(m, 2)
    Debug.Print m(2, 3); m(1, 1)
End Sub
