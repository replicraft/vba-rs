' INV: vbal.6.1.2.5.1.6, func.filelen
' SOURCE: spec ms-vbal 6.1.2.5.1.6 FileLen returns Long length in bytes
' EXPECT: len=3
' EXPECT: type=Long
' EXPECT: empty=0
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim f As Integer
    f = FreeFile
    Open "/fl.bin" For Binary As #f
    Put #f, , CByte(65)
    Put #f, , CByte(66)
    Put #f, , CByte(67)
    Close #f
    Debug.Print "len=" & CStr(FileLen("/fl.bin"))
    Debug.Print "type=" & TypeName(FileLen("/fl.bin"))
    Open "/fl0.bin" For Output As #f
    Close #f
    Debug.Print "empty=" & CStr(FileLen("/fl0.bin"))
End Sub
