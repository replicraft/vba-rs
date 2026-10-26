' INV: vbal.5.4.5.9, stmt.write
' SOURCE: spec ms-vbal 5.4.5.9 Date output forms: date only, date and time, time only
' EXPECT: [#2020-01-02#]
' EXPECT: [#2020-01-02 03:04:05#]
' EXPECT: [#03:04:05#]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/w.txt" For Output As #1
    Write #1, DateSerial(2020, 1, 2)
    Write #1, DateSerial(2020, 1, 2) + TimeSerial(3, 4, 5)
    Write #1, TimeSerial(3, 4, 5)
    Close #1
    Open "/t/w.txt" For Input As #1
    Do While Not EOF(1)
        Line Input #1, s
        Debug.Print "[" & s & "]"
    Loop
    Close #1
End Sub
