' INV: vbal.5.2.2
' SOURCE: spec ms-vbal 5.2.2 letter-range ascending covers all letters in span
' EXPECT: Integer Integer Integer Empty
DefInt A-C
Public Sub Main()
    Dim ax, bx, cx, dx
    Debug.Print TypeName(ax) & " " & TypeName(bx) & " " & TypeName(cx) & " " & TypeName(dx)
End Sub
