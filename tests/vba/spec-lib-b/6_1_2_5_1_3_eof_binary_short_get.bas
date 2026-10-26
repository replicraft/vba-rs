' INV: vbal.6.1.2.5.1.3, func.eof
' SOURCE: spec ms-vbal 6.1.2.5.1.3 EOF for Binary/Random is False until a Get cannot read an entire record
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: EOF on an Output file is error 54; EOF is True after the last full record
' EXPECT: g1=False
' EXPECT: g2=False
' EXPECT: g3=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim f As Integer, b As Byte
    f = FreeFile
    Open "/eb.bin" For Binary As #f
    Put #f, , CByte(1)
    Put #f, , CByte(2)
    Seek #f, 1
    Get #f, , b
    Debug.Print "g1=" & CStr(EOF(f))
    Get #f, , b
    Debug.Print "g2=" & CStr(EOF(f))
    On Error Resume Next
    Get #f, , b
    On Error GoTo 0
    Debug.Print "g3="& CStr(EOF(f))
    Close #f
End Sub
