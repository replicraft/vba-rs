' INV: stmt.erase
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/erase-statement
' EXPECT:  0 
Public Sub Main()
    Dim a(1 To 3) As Long
    a(2) = 9
    Erase a
    Debug.Print a(2)
End Sub
