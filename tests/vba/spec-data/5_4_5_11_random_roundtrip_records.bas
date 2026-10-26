' INV: vbal.5.4.5.11, stmt.put, stmt.get
' SOURCE: spec ms-vbal 5.4.5.12 Random: Get of a record returns the value Put into that record
' EXPECT: two one
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim a As String * 5, b As String * 5, r1 As String * 5, r2 As String * 5
    a = "one": b = "two"
    Open "/t/g.dat" For Random As #1 Len = 5
    Put #1, 1, a
    Put #1, 2, b
    Get #1, 2, r1
    Get #1, 1, r2
    Close #1
    Debug.Print Trim(r1) & " " & Trim(r2)
End Sub
