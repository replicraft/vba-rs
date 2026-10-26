' INV: vbal.6.1.2.5.1.10, func.seek
' SOURCE: spec ms-vbal 6.1.2.5.1.10 Seek in Random mode is the number of the next record
' EXPECT: start=1
' EXPECT: after3=4
' EXPECT: after1=2
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim f As Integer, v As Long
    f = FreeFile
    Open "/sr.rnd" For Random As #f Len = 4
    Debug.Print "start=" & CStr(Seek(f))
    v = 5
    Put #f, 3, v
    Debug.Print "after3=" & CStr(Seek(f))
    Get #f, 1, v
    Debug.Print "after1=" & CStr(Seek(f))
    Close #f
End Sub
