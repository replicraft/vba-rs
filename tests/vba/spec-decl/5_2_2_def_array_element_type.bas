' INV: vbal.5.2.2
' SOURCE: spec ms-vbal 5.2.3.1.1 implicitly typed array uses implicit element type
' EXPECT: Integer
' EXPECT: Integer
DefInt A
Dim afixed(2)
Dim arsz()
Public Sub Main()
    Debug.Print TypeName(afixed(0))
    ReDim arsz(2)
    Debug.Print TypeName(arsz(0))
End Sub
