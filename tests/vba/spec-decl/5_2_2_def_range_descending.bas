' INV: vbal.5.2.2
' SOURCE: spec ms-vbal 5.2.2 letter-range can be descending
' EXPECT: String String String Empty
DefStr C-A
Public Sub Main()
    Dim ax, bx, cx, dx
    Debug.Print TypeName(ax) & " " & TypeName(bx) & " " & TypeName(cx) & " " & TypeName(dx)
End Sub
