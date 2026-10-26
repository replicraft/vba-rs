' INV: stmt.redim
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/redim-statement
' EXPECT:  1  3  5 
Public Sub Main()
    Dim a() As Long
    ReDim a(1 To 3)
    a(2) = 5
    Debug.Print LBound(a); UBound(a); a(2)
End Sub
