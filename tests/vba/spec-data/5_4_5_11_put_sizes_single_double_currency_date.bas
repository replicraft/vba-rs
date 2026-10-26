' INV: vbal.5.4.5.11, stmt.put
' SOURCE: spec ms-vbal 5.4.5.11 Single is 4 bytes; Double, Currency and Date are 8 bytes
' EXPECT: 4 8 8 8
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As Single, d As Double, c As Currency, t As Date
    Dim n1 As Long, n2 As Long, n3 As Long, n4 As Long
    Open "/t/g1.bin" For Binary As #1
    Put #1, , s
    n1 = LOF(1)
    Close #1
    Open "/t/g2.bin" For Binary As #1
    Put #1, , d
    n2 = LOF(1)
    Close #1
    Open "/t/g3.bin" For Binary As #1
    Put #1, , c
    n3 = LOF(1)
    Close #1
    Open "/t/g4.bin" For Binary As #1
    Put #1, , t
    n4 = LOF(1)
    Close #1
    Debug.Print CStr(n1) & " " & CStr(n2) & " " & CStr(n3) & " " & CStr(n4)
End Sub
