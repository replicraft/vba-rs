' INV: stmt.chdrive
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/chdrive-statement
' EXPECT: ok
' GRANT: file-read
Public Sub Main()
    ChDrive "C"
    Debug.Print "ok"
End Sub
