' INV: stmt.redim
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/redim-statement
' EXPECT:  4  4 
Public Sub Main()
    Dim m() As Long
    ReDim m(1 To 2, 1 To 2)
    m(2, 2) = 4
    ReDim Preserve m(1 To 2, 1 To 4)
    Debug.Print m(2, 2); UBound(m, 2)
End Sub
