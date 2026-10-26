' INV: stmt.deftype
' SOURCE: spec ms-vbal 5.2.2 Implicit Definition Directives
' EXPECT: IntegerStringEmpty
DefInt A-C
DefStr S
Public Sub Main()
    Dim apple, sorted, zed
    Debug.Print TypeName(apple); TypeName(sorted); TypeName(zed)
End Sub
