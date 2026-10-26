' INV: stmt.close, stmt.open
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/close-statement
' EXPECT:  1 
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim f1 As Integer, f2 As Integer
    f1 = FreeFile: Open "/t/c1" For Output As #f1
    f2 = FreeFile: Open "/t/c2" For Output As #f2
    Close
    Debug.Print FreeFile
End Sub
