' INV: vbal.6.1.2.5.1.6, func.filelen
' SOURCE: spec ms-vbal 6.1.2.5.1.6 FileLen raises an error when PathName is not found
' EXPECT: raised=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim n As Long
    On Error Resume Next
    n = FileLen("/no/such/file.dat")
    Debug.Print "raised=" & CStr(Err.Number <> 0)
End Sub
