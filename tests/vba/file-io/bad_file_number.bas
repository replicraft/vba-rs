' INV: stmt.open, stmt.print
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/printstatement
' ERROR: 52
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Print #3, "x"
End Sub
