' INV: vbal.5.4.5.11, stmt.put, stmt.get
' SOURCE: spec ms-vbal 5.4.5.11 a Variant is written with its type descriptor; a String in random mode with a two-byte length; a record longer than Len is error 59
' EXPECT: 2 abc 7.5
' EXPECT: 5 xy Empty
' EXPECT: err59=True
' GRANT: file-read
' GRANT: file-write
Private Type Rec
    a As Variant
    s As String
    b As Variant
End Type

Public Sub Main()
    MkDir "/t"
    Dim r As Rec, q As Rec
    Open "/t/v.dat" For Random As #1 Len = 40
    r.a = 2
    r.s = "abc"
    r.b = 7.5
    Put #1, 1, r
    r.a = 5
    r.s = "xy"
    r.b = Empty
    Put #1, 2, r
    Get #1, 1, q
    Debug.Print q.a & " " & q.s & " " & q.b
    Get #1, 2, q
    Debug.Print q.a & " " & q.s & " " & TypeName(q.b)
    r.s = String(40, "z")
    On Error Resume Next
    Put #1, 3, r
    Debug.Print "err59=" & CStr(Err.Number = 59)
    Close #1
End Sub
