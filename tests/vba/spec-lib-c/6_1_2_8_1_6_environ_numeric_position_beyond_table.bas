' INV: vbal.6.1.2.8.1.6, func.environ
' SOURCE: spec ms-vbal 6.1.2.8.1.6 Environ numeric position without an entry returns a zero-length string
' EXPECT: len=0
' EXPECT: form=True
Public Sub Main()
    Debug.Print "len=" & Len(Environ(1000000))
    Dim s As String, i As Long, ok As Boolean
    ok = True
    ' every present entry has the form name=value; entries are numbered from 1 without gaps
    For i = 1 To 5
        s = Environ(i)
        If Len(s) = 0 Then Exit For
        If InStr(s, "=") = 0 Then ok = False
    Next i
    Debug.Print "form=" & ok
End Sub
