' INV: stmt.redim
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/redim-statement
' ERROR: 9
Public Sub Main()
    Dim m() As Long
    ReDim m(1 To 2, 1 To 2)
    ReDim Preserve m(1 To 3, 1 To 2)
End Sub
