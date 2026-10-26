' INV: vbal.5.4.5.1, stmt.open, stmt.put
' SOURCE: spec ms-vbal 5.4.5.1 Len is the sum of the sizes of the data types in a record
' EXPECT: 5 2.5
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim a As Long, b As Single, c As Long, d As Single
    a = 5: b = 2.5
    Open "/t/rf.dat" For Random As #1 Len = 8
    Put #1, 1, a
    Get #1, 1, c
    Close #1
    Open "/t/rf2.dat" For Random As #1 Len = 8
    Put #1, 1, b
    Get #1, 1, d
    Close #1
    Debug.Print CStr(c) & " " & CStr(d)
End Sub
