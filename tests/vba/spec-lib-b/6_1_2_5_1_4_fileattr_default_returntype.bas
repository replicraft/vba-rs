' INV: vbal.6.1.2.5.1.4, func.fileattr
' SOURCE: spec ms-vbal 6.1.2.5.1.4 FileAttr ReturnType defaults to 1 and result is Long
' XFAIL: wrong pending
' EXPECT: mode=2
' EXPECT: type=Long
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim f As Integer
    f = FreeFile
    Open "/fa2.dat" For Output As #f
    Debug.Print "mode=" & CStr(FileAttr(f))
    Debug.Print "type=" & TypeName(FileAttr(f))
    Close #f
End Sub
