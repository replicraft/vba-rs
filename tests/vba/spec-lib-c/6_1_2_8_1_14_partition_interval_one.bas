' INV: vbal.6.1.2.8.1.14, func.partition
' SOURCE: spec ms-vbal 6.1.2.8.1.14 Partition with Interval 1 gives number:number
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: Partition pads to Len(Stop) as the Learn page shows
' EXPECT: [  100:  100]
Public Sub Main()
    Debug.Print "[" & Partition(100, 0, 1000, 1) & "]"
End Sub
