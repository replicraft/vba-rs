' INV: vbal.6.1.2.5.1.6, func.filelen
' SOURCE: spec ms-vbal 6.1.2.5.1.6 FileLen of an open file is the size before it was opened
' XFAIL: wrong pending
' EXPECT: open=3
' EXPECT: closed=5
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim f As Integer
    f = FreeFile
    Open "/flo.bin" For Binary As #f
    Put #f, , CByte(1)
    Put #f, , CByte(2)
    Put #f, , CByte(3)
    Close #f
    Open "/flo.bin" For Binary As #f
    Put #f, 4, CByte(4)
    Put #f, 5, CByte(5)
    Debug.Print "open=" & CStr(FileLen("/flo.bin"))
    Close #f
    Debug.Print "closed=" & CStr(FileLen("/flo.bin"))
End Sub
