' INV: func.filedatetime, stmt.open
' SOURCE: unverified the in-memory file system stamps files with a fixed 2000-01-01 clock
' EXPECT:  2000  1  1 
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Open "/a.txt" For Output As #1: Close #1
    Dim d As Date
    d = FileDateTime("/a.txt")
    Debug.Print Year(d); Month(d); Day(d)
End Sub
