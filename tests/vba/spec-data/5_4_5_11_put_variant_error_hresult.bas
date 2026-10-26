' INV: vbal.5.4.5.11, stmt.put, stmt.get
' SOURCE: spec ms-vbal 5.4.5.11 Variant Error: descriptor 10 00 then the error code as an HRESULT (0x800A000D for error 13)
' EXPECT: len=6 10 0 | 13 0 10 128
' EXPECT: back=Error 13 True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim v As Variant, w As Variant, raw(0 To 5) As Byte, i As Integer, s As String
    v = CVErr(13)
    Open "/t/e.bin" For Binary As #1
    Put #1, , v
    Seek #1, 1
    Get #1, , raw
    s = "len=" & CStr(LOF(1))
    For i = 0 To 5
        If i = 2 Then s = s & " |"
        s = s & " " & CStr(raw(i))
    Next i
    Debug.Print s
    Seek #1, 1
    Get #1, , w
    Debug.Print "back=" & CStr(w) & " " & CStr(IsError(w))
    Close #1
End Sub
