' INV: vbal.5.2.2
' SOURCE: spec ms-vbal 5.2.2 def applies only to entities not explicitly typed
' EXPECT: Long Double Integer
DefInt A-Z
Public Sub Main()
    Dim ax As Long
    Dim bx#
    Dim cx
    Debug.Print TypeName(ax) & " " & TypeName(bx) & " " & TypeName(cx)
End Sub
