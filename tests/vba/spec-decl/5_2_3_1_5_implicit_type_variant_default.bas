' INV: vbal.5.2.3.1.5
' SOURCE: spec ms-vbal 5.2.3.1.5 implicit declared type is Variant, outside any def span
' EXPECT: Empty|Integer
DefInt A
Public Sub Main()
    Dim zed
    Dim ant
    Debug.Print TypeName(zed) & "|" & TypeName(ant)
End Sub
