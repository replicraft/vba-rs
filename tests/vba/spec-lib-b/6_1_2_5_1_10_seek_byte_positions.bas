' INV: vbal.6.1.2.5.1.10, func.seek
' SOURCE: spec ms-vbal 6.1.2.5.1.10 Seek: first byte is position 1; next-operation position
' EXPECT: out0=1
' EXPECT: out2=3
' EXPECT: in0=1
' EXPECT: app=3
' EXPECT: bin=4
' EXPECT: binseek=2
' EXPECT: type=Long
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim f As Integer, b As Byte
    f = FreeFile
    Open "/sk.txt" For Output As #f
    Debug.Print "out0=" & CStr(Seek(f))
    Print #f, "AB";
    Debug.Print "out2=" & CStr(Seek(f))
    Close #f
    Open "/sk.txt" For Input As #f
    Debug.Print "in0=" & CStr(Seek(f))
    Close #f
    Open "/sk.txt" For Append As #f
    Debug.Print "app=" & CStr(Seek(f))
    Close #f
    Open "/sk.bin" For Binary As #f
    Put #f, , CByte(1)
    Put #f, , CByte(2)
    Put #f, , CByte(3)
    Debug.Print "bin=" & CStr(Seek(f))
    Seek #f, 2
    Debug.Print "binseek=" & CStr(Seek(f))
    Debug.Print "type=" & TypeName(Seek(f))
    Close #f
End Sub
