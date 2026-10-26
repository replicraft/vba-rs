' INV: vbal.5.2.2
' SOURCE: spec ms-vbal 5.2.2 DefLngLng declared type
' EXPECT: LongLong
DefLngLng A
Public Sub Main()
    Dim ax
    Debug.Print TypeName(ax)
End Sub
