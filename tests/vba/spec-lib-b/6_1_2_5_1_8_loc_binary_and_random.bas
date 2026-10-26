' INV: vbal.6.1.2.5.1.8, func.loc
' SOURCE: spec ms-vbal 6.1.2.5.1.8 Loc: last byte (Binary) and last record (Random)
' EXPECT: bin=3
' EXPECT: binget=2
' EXPECT: rnd=3
' EXPECT: rndget=1
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim f As Integer, b As Byte, v As Long
    f = FreeFile
    Open "/lc.bin" For Binary As #f
    Put #f, , CByte(10)
    Put #f, , CByte(20)
    Put #f, , CByte(30)
    Debug.Print "bin=" & CStr(Loc(f))
    Seek #f, 1
    Get #f, , b
    Get #f, , b
    Debug.Print "binget=" & CStr(Loc(f))
    Close #f
    Open "/lc.rnd" For Random As #f Len = 4
    v = 7
    Put #f, 3, v
    Debug.Print "rnd=" & CStr(Loc(f))
    Get #f, 1, v
    Debug.Print "rndget=" & CStr(Loc(f))
    Close #f
End Sub
