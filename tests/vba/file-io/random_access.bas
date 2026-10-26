' INV: stmt.get, stmt.put, stmt.open, stmt.close
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/get-statement
' EXPECT:  2 cdef
' EXPECT:  1 [ab  ]
' GRANT: file-read
' GRANT: file-write
Type Rec
    ID As Long
    Name As String * 4
End Type
Public Sub Main()
    Dim f As Integer, r As Rec, q As Rec
    f = FreeFile
    MkDir "/t"
    Open "/t/r.dat" For Random As #f Len = 8
    r.ID = 1: r.Name = "ab"
    Put #f, 1, r
    r.ID = 2: r.Name = "cdef"
    Put #f, 2, r
    Get #f, 2, q
    Debug.Print q.ID; q.Name
    Get #f, 1, q
    Debug.Print q.ID; "[" & q.Name & "]"
    Close #f
End Sub
