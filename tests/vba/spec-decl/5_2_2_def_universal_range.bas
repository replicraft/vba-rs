' INV: vbal.5.2.2
' SOURCE: spec ms-vbal 5.2.2 universal-letter-range covers every identifier
' EXPECT: Integer Integer Integer
DefInt A-Z
Public Sub Main()
    Dim alpha, mid, zulu
    Debug.Print TypeName(alpha) & " " & TypeName(mid) & " " & TypeName(zulu)
End Sub
