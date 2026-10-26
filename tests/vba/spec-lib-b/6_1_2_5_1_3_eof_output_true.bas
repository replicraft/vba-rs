' INV: vbal.6.1.2.5.1.3, func.eof
' SOURCE: spec ms-vbal 6.1.2.5.1.3 EOF returns True for a file opened for Output
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: EOF on an Output file is error 54; EOF is True after the last full record
' EXPECT: eof=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim f As Integer
    f = FreeFile
    Open "/eo.txt" For Output As #f
    Debug.Print "eof=" & CStr(EOF(f))
    Close #f
End Sub
