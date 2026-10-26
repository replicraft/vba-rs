' INV: vbal.6.1.2.8.1.14, func.partition
' SOURCE: spec ms-vbal 6.1.2.8.1.14 Partition before-first range ends at Start-1, after-last range starts at Stop+1 (blanks ignored)
' EXPECT: :19
' EXPECT: 200:
' EXPECT: :99
' EXPECT: 1011:
' EXPECT: 100:
Public Sub Main()
    Debug.Print Replace(Partition(19, 20, 199, 10), " ", "")
    Debug.Print Replace(Partition(200, 20, 199, 10), " ", "")
    Debug.Print Replace(Partition(50, 100, 1010, 20), " ", "")
    Debug.Print Replace(Partition(1011, 100, 1010, 20), " ", "")
    Debug.Print Replace(Partition(100, 0, 99, 5), " ", "")
End Sub
