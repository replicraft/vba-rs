' INV: stmt.erase
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/erase-statement
' ERROR: 9
Public Sub Main()
    Dim a() As Long
    ReDim a(1 To 3)
    Erase a
    Debug.Print UBound(a)
End Sub
