' INV: stmt.open, stmt.print, stmt.close, func.lof
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/open-statement
' EXPECT:  0 
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim f As Integer
    f = FreeFile
    Open "/t/o.txt" For Output As #f
    Print #f, "long long text"
    Close #f
    Open "/t/o.txt" For Output As #f
    Close #f
    Debug.Print FileLen("/t/o.txt")
End Sub
