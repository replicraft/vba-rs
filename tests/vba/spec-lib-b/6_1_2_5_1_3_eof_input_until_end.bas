' INV: vbal.6.1.2.5.1.3, func.eof
' SOURCE: spec ms-vbal 6.1.2.5.1.3 EOF False until file position is at end (sequential Input)
' EXPECT: before=False
' EXPECT: after=True
' EXPECT: empty=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim f As Integer, s As String
    f = FreeFile
    Open "/ei.txt" For Output As #f
    Print #f, "abc"
    Close #f
    Open "/ei.txt" For Input As #f
    Debug.Print "before=" & CStr(EOF(f))
    Line Input #f, s
    Debug.Print "after=" & CStr(EOF(f))
    Close #f
    Open "/ei2.txt" For Output As #f
    Close #f
    Open "/ei2.txt" For Input As #f
    Debug.Print "empty=" & CStr(EOF(f))
    Close #f
End Sub
