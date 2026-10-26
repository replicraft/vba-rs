' INV: vbal.6.1.2.5.1.7, func.freefile
' SOURCE: spec ms-vbal 6.1.2.5.1.7 FreeFile returns next file number available, ranges 1-255 and 256-511
' EXPECT: type=Integer
' EXPECT: range0=True
' EXPECT: range1=True
' EXPECT: differs=True
' EXPECT: freed=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim f As Integer, g As Integer, h As Integer
    f = FreeFile
    Debug.Print "type=" & TypeName(f)
    Debug.Print "range0=" & CStr(FreeFile(0) >= 1 And FreeFile(0) <= 255)
    Debug.Print "range1=" & CStr(FreeFile(1) >= 256 And FreeFile(1) <= 511)
    Open "/ff.txt" For Output As #f
    g = FreeFile
    Debug.Print "differs=" & CStr(g <> f)
    Close #f
    h = FreeFile
    Debug.Print "freed=" & CStr(h = f)
End Sub
