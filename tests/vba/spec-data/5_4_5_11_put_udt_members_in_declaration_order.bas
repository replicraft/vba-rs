' INV: vbal.5.4.5.11, stmt.put
' SOURCE: spec ms-vbal 5.4.5.11 UDT members are written in declaration order
' EXPECT: len=6 1 0 2 0 0 0
' GRANT: file-read
' GRANT: file-write
Private Type T
    a As Integer
    b As Long
End Type
Public Sub Main()
    MkDir "/t"
    Dim x As T
    Dim p(5) As Byte, k As Long, s As String
    x.a = 1: x.b = 2
    Open "/t/g.bin" For Binary As #1
    Put #1, , x
    Seek #1, 1
    For k = 0 To 5
        Get #1, , p(k)
        s = s & " " & CStr(p(k))
    Next k
    Debug.Print "len=" & CStr(LOF(1)) & s
    Close #1
End Sub
