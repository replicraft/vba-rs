' INV: stmt.get, stmt.put, stmt.open, stmt.close
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/put-statement
' EXPECT:  258 xyz
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim f As Integer, a As Long, b As Long, s As String * 3, t As String * 3
    f = FreeFile
    Open "/t/g.bin" For Binary As #f
    a = 258: s = "xyz"
    Put #f, , a
    Put #f, , s
    Seek #f, 1
    Get #f, , b
    Get #f, , t
    Close #f
    Debug.Print b; t
End Sub
