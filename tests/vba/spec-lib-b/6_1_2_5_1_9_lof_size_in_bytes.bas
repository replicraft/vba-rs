' INV: vbal.6.1.2.5.1.9, func.lof
' SOURCE: spec ms-vbal 6.1.2.5.1.9 LOF returns Long size in bytes of an open file
' EXPECT: emptylof=0
' EXPECT: lof=4
' EXPECT: type=Long
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim f As Integer
    f = FreeFile
    Open "/lo.bin" For Binary As #f
    Debug.Print "emptylof=" & CStr(LOF(f))
    Put #f, , CByte(1)
    Put #f, , CByte(2)
    Put #f, , CByte(3)
    Put #f, , CByte(4)
    Debug.Print "lof=" & CStr(LOF(f))
    Debug.Print "type=" & TypeName(LOF(f))
    Close #f
End Sub
