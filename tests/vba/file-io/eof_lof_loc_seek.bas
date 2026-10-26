' INV: func.eof, func.lof, func.loc, func.seek, stmt.seek, stmt.open
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/seek-statement
' EXPECT:  3  3  4 
' EXPECT:  66  2 False
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim f As Integer, b As Byte
    f = FreeFile
    Open "/t/b.bin" For Binary As #f
    Put #f, , CByte(65)
    Put #f, , CByte(66)
    Put #f, , CByte(67)
    Debug.Print LOF(f); Loc(f); Seek(f)
    Seek #f, 2
    Get #f, , b
    Debug.Print b; Loc(f); EOF(f)
    Close #f
End Sub
