' INV: vbal.6.1.2.5.1.5, func.filedatetime
' SOURCE: spec ms-vbal 6.1.2.5.1.5 FileDateTime returns a Date; error when not found
' EXPECT: type=Date
' EXPECT: raised=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim f As Integer, d As Variant
    f = FreeFile
    Open "/fdt.txt" For Output As #f
    Close #f
    Debug.Print "type=" & TypeName(FileDateTime("/fdt.txt"))
    On Error Resume Next
    d = FileDateTime("/no/such.txt")
    Debug.Print "raised=" & CStr(Err.Number <> 0)
End Sub
