' INV: vbal.6.1.2.8.1.14, func.partition
' SOURCE: spec ms-vbal 6.1.2.8.1.14 Partition ranges lowervalue:uppervalue, padded to len(Stop)+1 on each side of the colon
' XFAIL: wrong pending
' EXPECT: [  0:  4]
' EXPECT: [  5:  9]
' EXPECT: [ 95: 99]
' EXPECT: [  20:  29]
' EXPECT: [ 190: 199]
Public Sub Main()
    Debug.Print "[" & Partition(0, 0, 99, 5) & "]"
    Debug.Print "[" & Partition(7, 0, 99, 5) & "]"
    Debug.Print "[" & Partition(99, 0, 99, 5) & "]"
    Debug.Print "[" & Partition(25, 20, 199, 10) & "]"
    Debug.Print "[" & Partition(199, 20, 199, 10) & "]"
End Sub
