' INV: vbal.6.1.2.8.1.14, func.partition
' SOURCE: spec ms-vbal 6.1.2.8.1.14 Partition last range extends to Stop when Interval does not divide the span
' XFAIL: wrong pending
' EXPECT: [  100:  119]
' EXPECT: [ 1000: 1010]
' EXPECT: [ 1000: 1010]
Public Sub Main()
    Debug.Print "[" & Partition(100, 100, 1010, 20) & "]"
    Debug.Print "[" & Partition(1000, 100, 1010, 20) & "]"
    Debug.Print "[" & Partition(1010, 100, 1010, 20) & "]"
End Sub
