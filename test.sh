#compile sample
flowc -C flowr/src/cli -c -d flowsamples/sequence-of-sequences
while true;
do
  # run the sample, sending number sequence from stdout to result.txt and logs to log.txt
  flowr -v debug -t 2 -n flowsamples/sequence-of-sequences > result.txt 2> log.txt
  diff result.txt success.txt
  if [ $? -eq 1 ]
  then
    mv log.txt fail.txt
    echo "result.txt does not match success.txt, see fail.txt for logs"
    exit 1
  else
    mv log.txt pass.txt
    echo "passed, see log in pass.txt"
  fi
done
