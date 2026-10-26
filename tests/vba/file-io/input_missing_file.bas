' INV: stmt.open
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/open-statement
' ERROR: 53
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim f As Integer
    f = FreeFile
    Open "/t/missing.txt" For Input As #f
End Sub
