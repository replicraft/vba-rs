' INV: vbal.5.4.5.11, stmt.put, stmt.get
' SOURCE: spec ms-vbal 5.4.5.11 Variant Decimal: descriptor 14 00 then the 16 bytes of MS-OAUT 2.2.26 (reserved, scale, sign, high 32 bits, low 64 bits)
' EXPECT: len=18 14 0 | 0 0 1 128 | 0 0 0 0 | 15 0 0 0 0 0 0 0
' EXPECT: back=-1.5 Decimal
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim v As Variant, w As Variant, raw(0 To 17) As Byte, i As Integer, s As String
    v = CDec(-1.5)
    Open "/t/d.bin" For Binary As #1
    Put #1, , v
    Seek #1, 1
    Get #1, , raw
    s = "len=" & CStr(LOF(1))
    For i = 0 To 17
        If i = 2 Or i = 6 Or i = 10 Then s = s & " |"
        s = s & " " & CStr(raw(i))
    Next i
    Debug.Print s
    Seek #1, 1
    Get #1, , w
    Debug.Print "back=" & CStr(w) & " " & TypeName(w)
    Close #1
End Sub
