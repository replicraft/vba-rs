' INV: vbal.6.1.2.5.2.3, stmt.filecopy
' SOURCE: spec ms-vbal 6.1.2.5.2.3 FileCopy copies a file; source remains
' EXPECT: destlen=3
' EXPECT: srcstill=3
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim f As Integer
    f = FreeFile
    Open "/src.bin" For Binary As #f
    Put #f, , CByte(1)
    Put #f, , CByte(2)
    Put #f, , CByte(3)
    Close #f
    FileCopy "/src.bin", "/dst.bin"
    Debug.Print "destlen=" & CStr(FileLen("/dst.bin"))
    Debug.Print "srcstill=" & CStr(FileLen("/src.bin"))
End Sub
