' INV: stmt.open
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/open-statement
' ERROR: 55
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/x" For Output As #1
    Open "/t/y" For Output As #1
End Sub
