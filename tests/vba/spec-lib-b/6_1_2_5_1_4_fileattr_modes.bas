' INV: vbal.6.1.2.5.1.4, func.fileattr
' SOURCE: spec ms-vbal 6.1.2.5.1.4 FileAttr mode values for ReturnType 1
' EXPECT: out=2
' EXPECT: in=1
' EXPECT: app=8
' EXPECT: bin=32
' EXPECT: rnd=4
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim f As Integer
    f = FreeFile
    Open "/fa.dat" For Output As #f
    Debug.Print "out=" & CStr(FileAttr(f, 1))
    Close #f
    Open "/fa.dat" For Input As #f
    Debug.Print "in=" & CStr(FileAttr(f, 1))
    Close #f
    Open "/fa.dat" For Append As #f
    Debug.Print "app=" & CStr(FileAttr(f, 1))
    Close #f
    Open "/fa.dat" For Binary As #f
    Debug.Print "bin=" & CStr(FileAttr(f, 1))
    Close #f
    Open "/fa.dat" For Random As #f Len = 4
    Debug.Print "rnd=" & CStr(FileAttr(f, 1))
    Close #f
End Sub
