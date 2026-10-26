' INV: stmt.erase, stmt.redim
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/erase-statement
' EXPECT:  4 
Public Sub Main()
    Dim a() As Long
    ReDim a(2)
    Erase a
    ReDim a(4)
    Debug.Print UBound(a)
End Sub
