' INV: vbal.5.2.2
' SOURCE: spec ms-vbal 5.2.2 def-directive with several letter-specs
' EXPECT: Long Long Long Empty
DefLng A, C-D
Public Sub Main()
    Dim ax, cx, dx, bx
    Debug.Print TypeName(ax) & " " & TypeName(cx) & " " & TypeName(dx) & " " & TypeName(bx)
End Sub
