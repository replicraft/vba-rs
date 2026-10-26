' INV: vbal.6.1.2.5.2.6, stmt.rmdir
' SOURCE: spec ms-vbal 6.1.2.5.2.6 RmDir raises an error on a directory containing files
' EXPECT: raised=True
' EXPECT: still=full
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim f As Integer
    MkDir "/full"
    f = FreeFile
    Open "/full/x.txt" For Output As #f
    Close #f
    On Error Resume Next
    RmDir "/full"
    Debug.Print "raised=" & CStr(Err.Number <> 0)
    On Error GoTo 0
    Debug.Print "still=" & Dir("/full", vbDirectory)
End Sub
