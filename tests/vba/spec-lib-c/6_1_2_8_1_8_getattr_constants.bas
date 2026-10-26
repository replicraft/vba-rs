' INV: vbal.6.1.2.8.1.8, func.getattr
' SOURCE: spec ms-vbal 6.1.2.8.1.8 VbFileAttribute values
' EXPECT: 0 1 2 4 16 32
Public Sub Main()
    Debug.Print vbNormal & " " & vbReadOnly & " " & vbHidden & " " & vbSystem & " " & vbDirectory & " " & vbArchive
End Sub
